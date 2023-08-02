use std::fs::File;
use std::io::{self, Write};
use std::marker::PhantomData;
use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use bevy::prelude::*;
use bevy::utils::HashMap;
use crossbeam_utils::CachePadded;
use parking_lot::Mutex;
use serde::{Deserialize, Serialize};

pub fn new_progress_tracker<T>(
    save_path: Option<impl Into<PathBuf>>,
    data: Option<&[u8]>,
) -> (ProgressReader<T>, ProgressWriter<T>) {
    let tracker = Arc::new(ProgressTracker::new(save_path.map(|v| v.into()), data));
    let reader = ProgressReader {
        tracker: tracker.clone(),
        marker: PhantomData,
    };
    let writer = ProgressWriter {
        tracker,
        marker: PhantomData,
    };
    (reader, writer)
}

pub struct ProgressReader<T> {
    tracker: Arc<ProgressTracker>,
    marker: PhantomData<T>,
}

impl<T: Stage> ProgressReader<T> {
    pub fn stage(&self) -> T {
        match T::try_from(self.tracker.get_stage()) {
            Ok(v) => v,
            Err(_) => unreachable!(),
        }
    }

    pub fn percentage(&self) -> f32 {
        self.tracker.get_progress() * 100.0
    }
}

pub struct ProgressWriter<T> {
    tracker: Arc<ProgressTracker>,
    marker: PhantomData<T>,
}

impl<T: Stage> ProgressWriter<T> {
    pub fn set_stage(&mut self, stage: T) {
        self.tracker.set_stage(stage.into())
    }

    pub fn multi_task<R>(
        &mut self,
        num_subtasks: usize,
        callback: impl FnOnce(ProgressTask<'_>) -> R,
    ) -> R {
        self.tracker.begin_task(num_subtasks.try_into().unwrap());
        let res = callback(ProgressTask {
            tracker: &self.tracker,
        });
        self.tracker.end_task();
        res
    }

    pub fn task<R>(&mut self, callback: impl FnOnce() -> R) -> R {
        self.tracker.begin_task(1);
        let res = callback();
        self.tracker.end_task();
        res
    }

    pub fn finish(&mut self) {
        self.tracker.finish();
    }
}

pub struct ProgressTask<'a> {
    tracker: &'a ProgressTracker,
}

impl ProgressTask<'_> {
    pub fn subtask_completed(&self) {
        self.tracker.subtask_completed();
    }
}

pub trait Stage: Copy + Into<u32> + TryFrom<u32> {}

impl<T: Copy + Into<u32> + TryFrom<u32>> Stage for T {}

#[macro_export]
macro_rules! define_stages {
    (pub enum $ty:ident { $($name:ident => $message:expr,)* }) => {
        #[derive(Debug, Clone, Copy)]
        #[repr(u32)]
        pub enum $ty {
            $($name,)*
        }

        impl $ty {
            pub fn message(&self) -> &'static str {
                match self {
                    $( Self::$name => $message, )*
                }
            }
        }

        impl From<$ty> for u32 {
            fn from(v: $ty) -> u32 {
                v as u32
            }
        }

        impl TryFrom<u32> for $ty {
            type Error = &'static str;

            fn try_from(v: u32) -> Result<Self, Self::Error> {
                $( if v == $ty::$name as u32 {
                    return Ok($ty::$name);
                } )*

                Err("invalid stage")
            }
        }
    }
}

struct ProgressTracker {
    stage: CachePadded<AtomicU32>,
    counter: CachePadded<AtomicU64>,
    num_subtasks: CachePadded<AtomicU32>,
    progress: CachePadded<AtomicU32>,
    samples: Option<Mutex<RuntimeSamples>>,
    baked_samples: BakedSamples,
}

impl ProgressTracker {
    fn new(save_path: Option<PathBuf>, data: Option<&[u8]>) -> ProgressTracker {
        ProgressTracker {
            stage: CachePadded::new(AtomicU32::new(0)),
            counter: CachePadded::new(AtomicU64::new(0)),
            num_subtasks: CachePadded::new(AtomicU32::new(0)),
            progress: CachePadded::new(AtomicU32::new(0)),
            samples: save_path.map(|path| Mutex::new(RuntimeSamples::new(path))),
            baked_samples: data.map(|v| BakedSamples::load(v)).unwrap_or_default(),
        }
    }

    fn set_stage(&self, stage: u32) {
        self.stage.store(stage, Ordering::Relaxed)
    }

    fn get_stage(&self) -> u32 {
        self.stage.load(Ordering::Relaxed)
    }

    fn get_num_subtasks(&self) -> u32 {
        self.num_subtasks.load(Ordering::Relaxed)
    }

    fn get_task_subtask(&self) -> (u32, u32) {
        let counter = self.counter.load(Ordering::Relaxed);
        ((counter >> 32) as u32, counter as u32)
    }

    fn get_old_progress(&self) -> f32 {
        (self.progress.load(Ordering::Relaxed) as f32) / (u32::MAX as f32)
    }

    fn get_progress(&self) -> f32 {
        let old_progress = self.get_old_progress();

        let stage = self.get_stage();
        let (task, subtask) = self.get_task_subtask();
        let num_subtasks = self.get_num_subtasks();

        let Some(stage_range) = self.baked_samples.stages.get(&stage) else {
            return old_progress;
        };

        let Some(task_range) = self.baked_samples.tasks.get(&(stage, task)) else {
            return old_progress;
        };

        let task_progress = (subtask as f32) / (num_subtasks as f32);
        let stage_progress = task_range.start + task_progress * (task_range.end - task_range.start);
        let progress = stage_range.start + stage_progress * (stage_range.end - stage_range.start);

        self.progress.store(
            (progress.max(old_progress) * (u32::MAX as f32)) as u32,
            Ordering::Relaxed,
        );

        progress
    }

    fn begin_task(&self, num_subtasks: u32) {
        let counter = self.counter.fetch_and(!((1 << 32) - 1), Ordering::Relaxed);
        self.num_subtasks.store(num_subtasks, Ordering::Relaxed);
        if let Some(samples) = &self.samples {
            Self::sample_begin_task(samples, &self.stage, counter);
        }
    }

    fn subtask_completed(&self) {
        self.counter.fetch_add(1, Ordering::Relaxed);
    }

    fn end_task(&self) {
        let counter = self.counter.fetch_add(1 << 32, Ordering::Relaxed);
        if let Some(samples) = &self.samples {
            Self::sample_end_task(samples, &self.stage, counter);
        }
    }

    #[inline(never)]
    fn sample_begin_task(samples: &Mutex<RuntimeSamples>, stage: &AtomicU32, counter: u64) {
        let instant = Instant::now();
        let mut samples = samples.lock();

        let task_idx = (counter >> 32) as u32;
        let stage_idx = stage.load(Ordering::Relaxed);

        let stage = samples.tasks.entry((stage_idx, task_idx)).or_default();
        stage.begin_instant = Some(instant);
    }

    #[inline(never)]
    fn sample_end_task(samples: &Mutex<RuntimeSamples>, stage: &AtomicU32, counter: u64) {
        let mut samples = samples.lock();

        let task_idx = (counter >> 32) as u32;
        let stage_idx = stage.load(Ordering::Relaxed);

        let Some(stage) = samples.tasks.get_mut(&(stage_idx, task_idx)) else {
            return;
        };

        stage.end_instant = Some(Instant::now());
    }

    fn finish(&self) {
        if let Some(samples) = &self.samples {
            let samples = samples.lock();
            let baked = samples.bake();
            if let Err(e) = baked.save(&samples.save_path) {
                error!("failed to save progress data: {e:?}");
            }
        }
    }
}

struct RuntimeSamples {
    tasks: HashMap<(u32, u32), TaskSamples>,
    save_path: PathBuf,
}

impl RuntimeSamples {
    fn new(save_path: PathBuf) -> RuntimeSamples {
        RuntimeSamples {
            tasks: HashMap::default(),
            save_path,
        }
    }

    fn bake(&self) -> BakedSamples {
        let it = self.tasks.keys();
        let mut stages = it.map(|&(stage, _)| stage).collect::<Vec<u32>>();
        stages.sort();
        stages.dedup();

        let it = self.tasks.iter();
        let task_durations = it
            .map(
                |(&key, samples)| match (samples.begin_instant, samples.end_instant) {
                    (Some(begin), Some(end)) => (key, end - begin),
                    _ => (key, Duration::ZERO),
                },
            )
            .collect::<HashMap<(u32, u32), Duration>>();

        let stage_durations = stages
            .iter()
            .map(|stage| {
                task_durations
                    .iter()
                    .filter(|((s, _), _)| s == stage)
                    .map(|(_, v)| v)
                    .sum::<Duration>()
            })
            .collect::<Vec<_>>();

        let total_secs = stage_durations.iter().sum::<Duration>().as_secs_f32();

        let mut start = 0.0;
        let it = stages.iter().zip(&stage_durations);
        let stage_ranges = it
            .map(|(&stage, duration)| {
                let end = start + duration.as_secs_f32() / total_secs;
                let range = start..end;
                start = end;
                (stage, range)
            })
            .collect::<HashMap<u32, Range<f32>>>();

        let it = stages.iter().zip(&stage_durations);
        let task_ranges = it
            .flat_map(|(&stage, stage_duration)| {
                let mut tasks = task_durations
                    .iter()
                    .filter(|(&(s, _), _)| s == stage)
                    .collect::<Vec<_>>();
                tasks.sort_by_key(|(&stage, _)| stage);
                tasks.dedup_by_key(|(&stage, _)| stage);

                let total_secs = stage_duration.as_secs_f32();

                let mut start = 0.0;
                tasks
                    .iter()
                    .map(|(&task, duration)| {
                        let end = start + duration.as_secs_f32() / total_secs;
                        let range = start..end;
                        start = end;
                        (task, range)
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<HashMap<(u32, u32), Range<f32>>>();

        BakedSamples {
            stages: stage_ranges,
            tasks: task_ranges,
        }
    }
}

#[derive(Clone, Copy, Default)]
struct TaskSamples {
    begin_instant: Option<Instant>,
    end_instant: Option<Instant>,
}

#[derive(Default, Deserialize, Serialize)]
struct BakedSamples {
    stages: HashMap<u32, Range<f32>>,
    tasks: HashMap<(u32, u32), Range<f32>>,
}

impl BakedSamples {
    fn load(bytes: &[u8]) -> Self {
        rmp_serde::from_slice(bytes).unwrap_or_else(|e| {
            error!("invalid progress data: {e:?}");
            Self::default()
        })
    }

    fn save(&self, path: &Path) -> io::Result<()> {
        let mut writer = File::create(path)?;

        let data = rmp_serde::to_vec(self).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        writer.write_all(&data)?;

        Ok(())
    }
}

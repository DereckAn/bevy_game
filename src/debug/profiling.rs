//! Measurement-only hooks, compiled only with `--features tracy`.
use super::GpuToggles;
use crate::{
    core::{GameState, WorldKind, WorldSeed},
    voxel::{BaseChunk, ChunkGenerationTask, ChunkLoadQueue, ChunkMeshTask, LodChunk},
};
use bevy::prelude::*;
use std::{
    fs::OpenOptions,
    io::{BufWriter, Write},
};

const PHASES: [&str; 4] = ["warmup", "idle", "walk", "dig"];

#[derive(Resource, Default)]
struct Recording {
    output: Option<BufWriter<std::fs::File>>,
    phase: usize,
    samples: Vec<f64>,
    last_report: f64,
    state: String,
}

pub(super) fn install(app: &mut App) {
    let mut recording = Recording::default();
    if let Some(path) = std::env::var_os("BEVY_PROFILE_CSV") {
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => {
                let mut writer = BufWriter::new(file);
                if writeln!(writer, "elapsed_s,frame_ms,state,phase").is_ok() {
                    recording.output = Some(writer);
                    info!("Frame CSV: {:?}", path);
                } else {
                    warn!("Cannot write frame CSV header: {:?}", path);
                }
            }
            Err(error) => warn!(
                "Cannot create frame CSV {:?}: {} (existing files are preserved)",
                path, error
            ),
        }
    }
    app.insert_resource(recording)
        .add_systems(Last, record_frames);
}

// Nearest-rank percentiles; inputs must be sorted and nonempty.
fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    sorted[((sorted.len() as f64 * fraction).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len() - 1)]
}

#[allow(clippy::too_many_arguments)]
fn record_frames(
    time: Res<Time<Real>>,
    keys: Res<ButtonInput<KeyCode>>,
    state: Res<State<GameState>>,
    seed: Res<WorldSeed>,
    kind: Res<WorldKind>,
    toggles: Res<GpuToggles>,
    daylight: Option<Res<crate::sky::TimeOfDay>>,
    queue: Option<Res<ChunkLoadQueue>>,
    real: Query<(), With<BaseChunk>>,
    lod: Query<(), With<LodChunk>>,
    generating: Query<(), With<ChunkGenerationTask>>,
    meshing: Query<(), With<ChunkMeshTask>>,
    cameras: Query<&GlobalTransform, With<Camera3d>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut recording: ResMut<Recording>,
) {
    let now = time.elapsed_secs_f64();
    let state_label = format!("{:?}", state.get());
    let boundary = keys.just_pressed(KeyCode::F8) || recording.state != state_label;
    if keys.just_pressed(KeyCode::F8) {
        recording.phase = (recording.phase + 1) % PHASES.len();
    }
    if boundary {
        recording.samples.clear();
        recording.last_report = now;
        recording.state = state_label.clone();
        info!(
            "PROFILE segment: state={} phase={} seed={} world={:?}",
            state_label, PHASES[recording.phase], seed.0, *kind
        );
    }
    // Delta covers the previous frame. Skip the transition frame to avoid
    // attributing a menu/loading interval to the newly selected segment.
    let ms = time.delta_secs_f64() * 1000.0;
    if !boundary && ms > 0.0 {
        recording.samples.push(ms);
        let phase = PHASES[recording.phase];
        if let Some(writer) = recording.output.as_mut() {
            if let Err(error) = writeln!(writer, "{now:.6},{ms:.6},{state_label},{phase}") {
                warn!("Frame CSV recording disabled: {}", error);
                recording.output = None;
            }
        }
    }
    if now - recording.last_report < 1.0 || recording.samples.is_empty() {
        return;
    }
    let _span = info_span!("profiling_report_and_csv_flush").entered();
    recording.last_report = now;
    recording.samples.sort_by(f64::total_cmp);
    let samples = &recording.samples;
    info!("PROFILE window: phase={} frames={} p50_ms={:.3} p95_ms={:.3} p99_ms={:.3} max_ms={:.3} over_16_67={} over_33_33={} real={} lod={} generating={} meshing={} queued={} unload={} shadows={} lod_enabled={} sky={}",
        PHASES[recording.phase], samples.len(), percentile(samples, 0.5), percentile(samples, 0.95), percentile(samples, 0.99), samples.last().unwrap(),
        samples.iter().filter(|&&v| v > 16.67).count(), samples.iter().filter(|&&v| v > 33.33).count(),
        real.count(), lod.count(), generating.count(), meshing.count(), queue.as_ref().map_or(0, |q| q.to_load.len()), queue.as_ref().map_or(0, |q| q.to_unload.len()), toggles.shadows, toggles.lod_tiles, toggles.sky_effects);
    if let Some(daylight) = daylight {
        info!(
            "PROFILE daylight: fraction={} running={} day_length_s={} ",
            daylight.fraction, daylight.running, daylight.day_length_secs
        );
    }
    if let Ok(camera) = cameras.single() {
        info!(
            "PROFILE camera: position={:?} forward={:?}",
            camera.translation(),
            camera.forward()
        );
    }
    if let Ok(window) = windows.single() {
        info!(
            "PROFILE window: pixels={}x{} scale={} present_mode={:?}",
            window.physical_width(),
            window.physical_height(),
            window.scale_factor(),
            window.present_mode
        );
    }
    recording.samples.clear();
    if let Some(writer) = recording.output.as_mut() {
        if let Err(error) = writer.flush() {
            warn!("Frame CSV recording disabled: {}", error);
            recording.output = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recorder_resets_samples_at_segment_boundaries() {
        let mut app = App::new();
        app.insert_resource(Time::<Real>::default())
            .insert_resource(ButtonInput::<KeyCode>::default())
            .insert_resource(State::new(GameState::MainMenu))
            .insert_resource(WorldSeed::TEST)
            .insert_resource(WorldKind::default())
            .insert_resource(GpuToggles::default())
            .init_resource::<Recording>()
            .add_systems(Last, record_frames);
        app.world_mut()
            .resource_mut::<Time<Real>>()
            .advance_by(std::time::Duration::from_millis(16));
        app.update();
        assert!(app.world().resource::<Recording>().samples.is_empty());
        app.update();
        assert_eq!(app.world().resource::<Recording>().samples, vec![16.0]);
        app.world_mut()
            .resource_mut::<ButtonInput<KeyCode>>()
            .press(KeyCode::F8);
        app.update();
        let recording = app.world().resource::<Recording>();
        assert_eq!(recording.phase, 1);
        assert!(recording.samples.is_empty());
    }

    #[test]
    fn nearest_rank_percentiles_include_tail() {
        assert_eq!(percentile(&[7.0], 0.99), 7.0);
        let samples: Vec<_> = (1..=100).map(f64::from).collect();
        assert_eq!(percentile(&samples, 0.5), 50.0);
        assert_eq!(percentile(&samples, 0.95), 95.0);
        assert_eq!(percentile(&samples, 0.99), 99.0);
    }
}

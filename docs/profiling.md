# Measurement baseline — 2026-10-30

Capture: `/Users/laruina/Desktop/test.tracy`, exported using Tracy 0.14.1.
Hardware reported in capture: Apple M3 Max, Metal, 64 GiB memory.
The TEST world starts at 3.525 s; menu reentry is at 264.715 s.

| CPU zone | Calls | Mean | p95 | Maximum |
| --- | ---: | ---: | ---: | ---: |
| update_near_coverage | 16,614 | 5.875 ms | 9.382 ms | 13.100 ms |
| mark_ready_near_entities | 21,298 | 3.701 ms | 9.087 ms | 13.325 ms |

These are inclusive CPU zone durations, including possible mutex waits. The two
systems run on different threads: their durations must not be added as a frame
cost. Additional spans now separate lock acquisition, ownership construction,
mask/visibility work, and render readiness computation.

Worker chunk generation averages 1.231 ms over 61,070 calls; meshing averages
0.116 ms over 61,067 calls. Worker totals are not main-thread frame cost.
Coverage/readiness is the first CPU investigation priority based on this capture.
No optimization has been applied as part of this measurement update.

The capture contains no GPU timing events. Render schedule or `prepare_windows`
time is not a GPU duration: surface acquisition and synchronization can contribute.
GPU bottlenecks, including raycast tree shader cost, remain unmeasured. Use a Metal
GPU capture/System Trace if GPU timestamps are unavailable in the Bevy/Tracy setup.
The new instrumentation does not enable GPU timestamp queries.

Across the entire mixed capture, consecutive render-finished messages have a
median interval of 11.951 ms, p95 31.872 ms, and p99 39.643 ms. This includes menus,
loading, motion, digging, and graphics toggles, so it is not a steady gameplay
baseline. Several toggle experiments lasted less than one second. F4 changes
coverage ownership as well as LOD visibility: it does not isolate tree rendering.

## Repeatable next capture

Run from the repository root, using a fresh CSV filename each time:

```sh
BEVY_PROFILE_CSV=/tmp/bevy-profile-01.csv cargo run --release --features tracy
```

Connect Tracy and record as before. Use TEST (seed 1337), the same window pixel
size and camera route, and keep F3/F4/F5 unchanged for the baseline. Record the
commit and local diff, power mode, and whether the machine is thermally warm.
The Tracy build uses continuous updates and disables vsync; compare like builds.

1. Start in `warmup`; wait for near and far task/queue counters to settle.
2. Press F8 for `idle`; stand still facing the same forest for at least 30 seconds.
3. Press F8 for `walk`; follow a repeatable route for at least 60 seconds.
4. Press F8 for `dig`; dig in one location for at least 30 seconds.
5. Press F8 to return to `warmup`. Save the Tracy capture and CSV.

F8 labels observations only; it does not move the camera or control gameplay.
State changes generate markers and reset the reporting window but preserve the
selected label. Transition frames are omitted from CSV because their delta
belongs to the preceding state. Return the label to warmup before a new run.
The day/night simulation still advances; match time of day for graphics A/B runs.

CSV rows contain raw app-frame intervals from `Time<Real>`, not GPU durations.
Use all rows within a state/phase to calculate whole-segment percentiles; do not
average the once-per-second percentiles. Tracy messages report nearest-rank
p50/p95/p99, maximum, frames exceeding 16.67/33.33 ms, chunk/task/queue counts,
far tile counts, graphics toggles, camera position/direction, time of day, and window settings.
Per-second summaries can cover less than a full second after segment transitions.
CSV output is optional, buffered, and flushed once per report; existing files are
never overwritten. A forced kill can lose the final buffered rows. Reporting and
I/O have overhead, with a dedicated report span for inspecting it. All new hooks
are compiled only with `tracy`; the frame-time display unit fix applies always.

Additional CPU zones cover far tile generation, column LOD construction, tree
proxy construction, and collider construction. No gameplay scheduling, caching,
LOD distances, shader behavior, or old-code removal is changed.

Review the new idle/walk/dig measurements before selecting and authorizing an
optimization. In particular compare lock-wait versus work spans, whether spikes
coincide with queue growth, and whether frame tails persist when queues are empty.

## Second capture — test2.tracy

Reviewed `/Users/laruina/Desktop/test2.tracy` with the matching
`/tmp/bevy-profile-01.csv`. TEST seed 1337, Apple M3 Max/Metal, 2560×1440
physical pixels, scale 2, AutoNoVsync. Shadows, LOD tiles, and sky remained enabled
through all three labeled phases. GPU export still contains no timing events.

Frame percentiles below use individual CSV rows grouped by InGame phase, not an
average of window percentiles. Durations are between Tracy segment markers.

| Phase | Duration | Frames | Mean | p50 | p95 | p99 | Maximum |
| --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| idle | 29.22 s | 1,667 | 17.52 ms | 14.33 ms | 30.87 ms | 32.93 ms | 53.29 ms |
| walk | 71.36 s | 4,620 | 15.44 ms | 13.71 ms | 31.06 ms | 35.05 ms | 59.10 ms |
| dig | 133.58 s | 9,365 | 14.26 ms | 13.85 ms | 16.67 ms | 19.85 ms | 57.27 ms |

The idle position is effectively fixed but camera direction changes during the
segment. Walking and digging use different views/locations; digging also includes
streaming recovery and pauses (49 crater-operation messages). A faster mean in
one phase does not prove that activity is cheaper. The first mixed capture is not
a controlled before/after baseline for this recording.

### The important CPU finding

Mean per-call durations, grouped by Tracy segment start times:

| CPU span | Idle | Walk | Dig |
| --- | ---: | ---: | ---: |
| Coverage ownership construction | 7.297 ms | 7.016 ms | 8.316 ms |
| Coverage mask + visibility | 0.191 ms | 0.136 ms | 0.208 ms |
| Coverage lock acquisition | 0.0004 ms | 0.021 ms | 0.003 ms |
| Render readiness lock acquisition | 1.955 ms | 3.064 ms | 6.205 ms |
| Render readiness computation | 4.196 ms | 3.547 ms | 3.982 ms |

The renderer waits while the main world builds ownership under their shared
mutex. Lock acquisition spans measure elapsed acquisition time (including any
thread scheduling delay), not CPU instructions consumed spinning. These are
nested spans across overlapping threads; do not add them into a frame duration.
Ownership construction accounts for about 97% of the idle coverage system's
7.492 ms average. Mask texture work is not the main cost.

Every idle workload sample reports zero near/far queued or generating work, zero
meshing tasks, and zero pending unloads. Counts remain 22,502 BaseChunk entities,
9,448 LOD entities, and 2,500 live far tiles. Entity counts are not visible draw
calls. Expensive coverage/readiness work therefore persists after loading settles.

During walking, sampled load backlog reaches 23,715 and pending unloads 16,727.
These are outstanding queue entries, not completed jobs or proof of a memory
leak. Existing unload code processes at most 16 entries per frame. Investigate
streaming throughput and retained entities after the coverage change; do not
simply raise budgets and risk new frame spikes.

Across the entire capture, CPU construction means are small compared with
coverage: column LOD 0.0205 ms/call; tree proxy 0.00104 ms/call; far tile generation
0.4606 ms/job; chunk generation 1.1954 ms/job; chunk meshing 0.1104 ms/job; collider
construction 0.3156 ms/call. Jobs can overlap, collider spans can be nested, and
means alone do not rule out burst costs. These data do not justify making tree
proxy generation the first optimization. Shader cost is still unknown.
The profiling report/flush span averages 0.146 ms once per reporting interval,
with a 0.297 ms maximum; this does not include all instrumentation overhead.

### Recommended implementation order, awaiting authorization

1. Change coverage/readiness bookkeeping: update affected columns on chunk,
   mesh, visibility, and readiness changes; move expensive work outside the
   shared mutex and exchange coherent snapshots/results with short lock holds.
   Preserve mesh-version checks, pending GPU asset/pipeline retries, removal
   handling, and no-hole near/far transitions. Do not just skip stationary-camera
   frames: streaming, remeshing, and GPU readiness still change at rest.
2. Repeat this protocol and compare the same scene, counts, and camera direction.
   Verify reduced ownership work and render waiting, then inspect frame p95/p99.
   Do not promise an FPS increase equal to summed CPU-zone savings.
3. Review streaming backlog/retirement and obtain GPU timings separately before
   choosing further terrain/tree shader or LOD-quality changes.

There is enough CPU evidence to start the first scoped optimization when approved.
No optimization or old-code deletion was made during this review.

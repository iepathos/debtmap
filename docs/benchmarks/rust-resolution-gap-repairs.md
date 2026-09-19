# Rust resolution gap repairs: performance comparison

Measured on 2026-09-19 using debug builds of baseline `36d6f670` and the completed
repairs, with the same frozen inputs for both revisions. Neither CLI series
exceeds the requested runtime or memory investigation thresholds, and no
Criterion boundary case shows a regression above 10%. Substantial host contention
makes the repository comparison observational; its lower final median is not
evidence of a speedup.

## Inputs and method

The repository input is a clean detached worktree of `36d6f670` at
`/tmp/debtmap-gap-baseline`. Its tracked-file SHA-256 manifest digest is
`a7b57f4c2f5be8e094649696fa77eb6ffa567e6dc1ce79fa1da5c3a274d38b3a`.
The existing LCOV artifact is copied unchanged to its
`target/coverage/lcov.info`; its SHA-256 is
`fc302591a09e4529b62401b0a6637cf5afca0a8773535e715c9fa494af5b0249`.
Repository history is pinned by the detached worktree revision.

The synthetic input contains 401 parse-only Rust files, 10,001 LOC and 9,201
functions. Four hundred modules each contain a unit owner with one method, a
helper, a start function, and twenty cross-module step functions. The root
contains all module declarations and calls the last module. This workload is
recreated because the previous temporary benchmark inputs were cleaned. It is
frozen for this comparison, not assumed byte-identical to earlier measurements.
Its source-manifest SHA-256 is
`380bc96f02d34658e18cdcddadef0ebbd719b0a17564b70fb9a6612eea3dacc1`.
No Cargo build of the synthetic workspace is involved.

Each CLI series consists of one excluded warmup and five measured runs. Runs are
serial, with no concurrent compilation or test workloads. Wall time and peak RSS
come from macOS `/usr/bin/time -l`; RSS is reported in bytes and converted to MiB.
The existing `--profile` output supplies graph-building, history-preload and
function-scoring times. Phase medians are calculated independently and are not
additive. Raw output, profiles and build logs are retained locally under
`/tmp/debtmap-gap-perf`.

The machine uses an Apple M2 Pro, 16 GiB RAM, macOS 15.5 (24F74), and rustc 1.89.0.
Dependencies are resolved offline; no release builds are used.

## Commands

Build the baseline from the detached worktree:

```sh
CARGO_TARGET_DIR=/tmp/debtmap-gap-build cargo build --offline --bin debtmap
cp /tmp/debtmap-gap-build/debug/debtmap /tmp/debtmap-gap-baseline-bin
```

Build and freeze the final binary from the repaired main workspace:

```sh
cargo build --offline --bin debtmap
cp target/debug/debtmap /tmp/debtmap-gap-final-bin
```

From `/tmp/debtmap-gap-baseline`, execute an excluded warmup and then runs 1–5,
changing output filenames for each run:

```sh
/usr/bin/time -l /tmp/debtmap-gap-baseline-bin analyze . --no-tui --format json \
  --profile --profile-output /tmp/debtmap-gap-perf/baseline/run-1.profile.json \
  --context --lcov target/coverage/lcov.info \
  > /tmp/debtmap-gap-perf/baseline/run-1.json \
  2> /tmp/debtmap-gap-perf/baseline/run-1.stderr
```

Use the same command and working directory for the final binary, substituting
`/tmp/debtmap-gap-final-bin` and the final output directory. For synthetic runs,
use `/tmp/debtmap-gap-synthetic` as the working directory and replace
`--context --lcov target/coverage/lcov.info` with `--no-context-aware`.

The maintained Criterion boundary benchmark measures sequential and parallel
workspace construction at 199, 200, 201 and 401 files. Its source setup and initial
graph assertions are outside the measured closure:

```sh
CARGO_TARGET_DIR=/tmp/debtmap-gap-build cargo bench --offline --profile dev \
  --bench call_graph_bench -- rust_workspace_resolution \
  --sample-size 10 --warm-up-time 1 --measurement-time 1 --noplot \
  --save-baseline gap-baseline
```

The final command, run from the main workspace after its debug benchmark build,
shares the baseline results through Criterion's standard output-directory setting:

```sh
CRITERION_HOME=/tmp/debtmap-gap-build/criterion cargo bench --offline --profile dev \
  --bench call_graph_bench -- rust_workspace_resolution \
  --sample-size 10 --warm-up-time 1 --measurement-time 1 --noplot \
  --baseline gap-baseline
```

Criterion extends collection when ten samples require more than a second. No
bespoke runner or Python script is used.

The synthetic sources can be reproduced with these setup-only shell commands;
they are outside all timed regions:

```sh
mkdir -p /tmp/debtmap-gap-synthetic
for i in {0..399}; do printf 'mod m%s;\n' "$i"; done > /tmp/debtmap-gap-synthetic/lib.rs
printf 'pub fn root() { m399::start(); }\n' >> /tmp/debtmap-gap-synthetic/lib.rs
for i in {0..399}; do
  next=$(( (i + 1) % 400 ))
  {
    printf 'pub struct Owner;\nimpl Owner { pub fn run(&self) { crate::m%s::helper(); } }\npub fn helper() {}\npub fn start() { Owner.run(); step0(); }\n' "$next"
    for j in {0..19}; do
      printf 'pub fn step%s() { crate::m%s::helper(); }\n' "$j" "$next"
    done
  } > /tmp/debtmap-gap-synthetic/m${i}.rs
done
```

## Acceptance and limits

Investigate a median total-runtime increase above 10% or a peak-memory increase
above 20%. Report graph and scoring phases even when total time is within the
threshold. Check every CLI receipt for matching scope, zero failed files, and
successful requested coverage/context loading.

These debug measurements characterize the frozen inputs on one machine. The
smaller Criterion boundary workload and correctness fixtures do not establish
repository-scale performance or general Rust precision.

## Results and artifact identity

Baseline binary SHA-256:
`da8712a609549041371fea989b8a8a470dbf28a622b466830709937ef21031c7`.
Final binary SHA-256:
`b362990e88204f3d9f48ea1f2273b1266752d80af676d992e35b45f85ee3af04`.
The final implementation was uncommitted when measured. Its source-manifest
SHA-256 is `c39926d51aa12084aca18251b4f06ef9780724f5e85506810482e3255c848454`;
the corresponding baseline implementation digest is
`31797ae7f0a467a2d8305ff0bcee3c5ba863ce691f3c29542e0c4cb22a7782ff`.
These implementation manifests include tracked and untracked source files,
Rust files, and Cargo manifests, using this command from each workspace:

```sh
git ls-files --cached --others --exclude-standard -z -- src '*.rs' Cargo.toml Cargo.lock \
  | sort -z | xargs -0 shasum -a 256 | shasum -a 256
```

Both binaries, the final implementation, repository input, synthetic input, and
LCOV fingerprints were checked again after all measurements and were unchanged.
All repository receipts report 901 analyzed files, 333,725 LOC, 9,491 production
functions, 6,320 test functions, zero failed files, requested context, and loaded
LCOV. All synthetic receipts report 401 files, 10,001 LOC, 9,201 functions, and
zero failed files.

| Series | Run 1 s | Run 2 s | Run 3 s | Run 4 s | Run 5 s |
| --- | ---: | ---: | ---: | ---: | ---: |
| Repository baseline | 128.65 | 112.11 | 115.96 | 192.13 | 198.82 |
| Repository final | 113.01 | 114.13 | 119.43 | 115.63 | 113.68 |
| Synthetic baseline | 7.38 | 7.47 | 7.47 | 7.68 | 8.00 |
| Synthetic final | 7.49 | 7.45 | 7.57 | 7.43 | 7.70 |

| Series | Median total s | Graph s | History preload s | Function scoring s | Median peak MiB | Highest peak MiB |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| Repository baseline | 128.65 | 30.270 | 62.297 | 9.744 | 1002.17 | 1249.14 |
| Repository final | 114.13 | 28.908 | 51.645 | 8.719 | 1130.98 | 1154.33 |
| Synthetic baseline | 7.47 | 2.795 | — | 2.109 | 272.47 | 277.70 |
| Synthetic final | 7.49 | 2.853 | — | 2.072 | 267.78 | 282.06 |

The excluded baseline repository and synthetic warmups took 111.32 s and 7.60 s;
final warmups took 119.37 s and 7.47 s. Repository timings show substantial host
contention: during the last two baseline runs an unrelated macOS virtual-machine
process consumed roughly four
to eight CPU cores. No concurrent project builds or tests ran. Ordinary desktop
applications also remained active. History preload ranged from 50.133 s to
98.063 s; graph building ranged from 30.125 s to 51.183 s. These baseline results
must not be used to claim a performance improvement from a quieter final run.
The measurements remain observational; preserve raw spread and investigate
threshold breaches with phase attribution and matched repeats. The unrelated VM
also consumed roughly five CPU cores during the final warmup, so neither series
represents a fully isolated host.

Repository median total time is numerically 11.29% lower, but host noise prevents
attributing that difference to the repairs. Median peak RSS rises 12.85%, while
the highest peak falls 7.59%. Synthetic median time rises 0.27%, graph time rises
2.08%, median peak RSS falls 1.72%, and highest peak RSS rises 1.57%. None crosses
the 10% total-runtime or 20% memory threshold. No additional matched repeats were
triggered. The larger repository RSS spread should be read with the same host
contention caveat as its wall times.

| Criterion case | Baseline estimate ms (95% interval) | Final estimate ms (95% interval) | Estimated change |
| --- | ---: | ---: | ---: |
| Sequential 199 | 165.09 (164.60–165.76) | 163.93 (162.85–165.30) | −0.71% |
| Parallel 199 | 164.51 (163.86–165.22) | 165.35 (161.85–169.93) | +0.51% |
| Sequential 200 | 169.61 (167.09–172.82) | 168.04 (164.00–174.25) | −0.92% |
| Parallel 200 | 170.41 (167.38–174.48) | 163.62 (162.86–164.43) | −3.98% |
| Sequential 201 | 171.48 (168.09–176.54) | 168.41 (166.05–171.97) | −1.79% |
| Parallel 201 | 170.02 (168.89–171.34) | 166.45 (165.04–168.14) | −2.10% |
| Sequential 401 | 555.76 (553.88–557.76) | 562.78 (554.96–570.94) | +1.26% |
| Parallel 401 | 558.08 (555.03–561.61) | 546.29 (544.39–548.10) | −2.11% |

Criterion reports no statistically significant regression. It labels parallel
200 and parallel 401 as improved, parallel 201 as within its noise threshold,
and the remaining cases as having no detected change. These small-workload
results do not establish a repository-scale improvement.

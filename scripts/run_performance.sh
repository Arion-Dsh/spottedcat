#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
PERF_MANIFEST="${ROOT_DIR}/perf/Cargo.toml"
MODE="${1:-full}"
OUTPUT_DIR="${2:-${ROOT_DIR}/target/perf/run-$(date +%Y%m%d-%H%M%S)}"
UPDATE_HZ="${SPOT_PERF_UPDATE_HZ:-60}"

case "${MODE}" in
    quick)
        WARMUP_FRAMES="${SPOT_PERF_WARMUP_FRAMES:-30}"
        SAMPLE_FRAMES="${SPOT_PERF_SAMPLE_FRAMES:-180}"
        CPU_SAMPLES="${SPOT_CPU_BENCH_SAMPLES:-15}"
        ;;
    full)
        WARMUP_FRAMES="${SPOT_PERF_WARMUP_FRAMES:-300}"
        SAMPLE_FRAMES="${SPOT_PERF_SAMPLE_FRAMES:-1800}"
        CPU_SAMPLES="${SPOT_CPU_BENCH_SAMPLES:-50}"
        ;;
    *)
        echo "usage: scripts/run_performance.sh [quick|full] [output-dir]" >&2
        exit 2
        ;;
esac

mkdir -p "${OUTPUT_DIR}"
cd "${ROOT_DIR}"

echo "[spot][perf] building comparison tool"
cargo build --manifest-path "${PERF_MANIFEST}" --target-dir "${ROOT_DIR}/target" \
    --release --bin spot-perf-compare

echo "[spot][perf] running CPU microbenchmarks"
SPOT_CPU_BENCH_SAMPLES="${CPU_SAMPLES}" \
    cargo bench --manifest-path "${PERF_MANIFEST}" --target-dir "${ROOT_DIR}/target" \
    --bench cpu --features model-3d -- \
    --output "${OUTPUT_DIR}/cpu.csv"

run_render() {
    local scenario="$1"
    local bench="$2"
    local perf_scenario="$3"
    local features="${4:-}"
    local command=(cargo bench --manifest-path "${PERF_MANIFEST}" --target-dir "${ROOT_DIR}/target" --bench "${bench}")
    if [[ -n "${features}" ]]; then
        command+=(--features "${features}")
    fi
    echo "[spot][perf] running ${scenario}"
    SPOT_PROFILE_RENDER=1 \
    SPOT_PROFILE_GPU="${SPOT_PROFILE_GPU:-1}" \
    SPOT_PROFILE_MEMORY="${SPOT_PROFILE_MEMORY:-1}" \
    SPOT_PROFILE_WARMUP_FRAMES="${WARMUP_FRAMES}" \
    SPOT_PROFILE_SAMPLE_FRAMES="${SAMPLE_FRAMES}" \
    SPOT_PROFILE_REPORT_EVERY="${SPOT_PROFILE_REPORT_EVERY:-120}" \
    SPOT_PROFILE_EXIT_AFTER_SAMPLE=1 \
    SPOT_PROFILE_SCENARIO="${scenario}_${UPDATE_HZ}hz" \
    SPOT_PROFILE_CSV="${OUTPUT_DIR}/${scenario}.frames.csv" \
    SPOT_PROFILE_SUMMARY="${OUTPUT_DIR}/${scenario}.summary.csv" \
    SPOT_PRESENT_MODE="${SPOT_PRESENT_MODE:-auto_no_vsync}" \
    SPOT_PERF_UPDATE_HZ="${UPDATE_HZ}" \
    SPOT_PERF_SCENARIO="${perf_scenario}" \
        "${command[@]}"
}

for scenario in sprite_batch sprite_state_changes text_cached text_dynamic offscreen; do
    run_render "2d_${scenario}" render_2d "${scenario}"
done

run_render "3d_render_state" render_3d render_state model-3d
run_render "3d_shader_switches" render_3d shader_switches model-3d
run_render "3d_instancing_10000" render_3d instancing model-3d
run_render "3d_offscreen" render_3d offscreen model-3d

echo "[spot][perf] completed: ${OUTPUT_DIR}"
echo "[spot][perf] compare later with: target/release/spot-perf-compare BASELINE_DIR ${OUTPUT_DIR} --tolerance 10"

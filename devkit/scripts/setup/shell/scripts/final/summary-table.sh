#!/usr/bin/env bash
# Coreverse Bootstrap - renders the final summary table from all *.result files
# produced by the toolchain/dependency/cmake steps.
#
# Each .result file holds one or more blocks of:
#   TOOL=...
#   STATUS=...
#   VERSION=...
# separated by a blank line (dependency-parse scripts can report multiple rows).
# Sourced by bootstrap.sh; do not execute directly.

SCRIPT_DIR_SUMMARY="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck disable=SC1091
. "$SCRIPT_DIR_SUMMARY/../common/logger.sh"

# show_summary_table <results_dir>
# Prints the table and sets CV_SUMMARY_FAILED / CV_SUMMARY_WARNING (global ints).
show_summary_table() {
    local results_dir="$1"
    log_banner "Coreverse Bootstrap - Summary"

    local tools=() statuses=() versions=()
    local file cur_tool="" cur_status="" cur_version="" line

    for file in "$results_dir"/*.result; do
        [ -e "$file" ] || continue
        cur_tool=""; cur_status=""; cur_version=""
        while IFS= read -r line || [ -n "$line" ]; do
            case "$line" in
                TOOL=*) cur_tool="${line#TOOL=}" ;;
                STATUS=*) cur_status="${line#STATUS=}" ;;
                VERSION=*) cur_version="${line#VERSION=}" ;;
                "")
                    if [ -n "$cur_tool" ]; then
                        tools+=("$cur_tool"); statuses+=("$cur_status"); versions+=("$cur_version")
                        cur_tool=""; cur_status=""; cur_version=""
                    fi
                    ;;
            esac
        done < "$file"
        if [ -n "$cur_tool" ]; then
            tools+=("$cur_tool"); statuses+=("$cur_status"); versions+=("$cur_version")
        fi
    done

    local name_width=20 version_width=15
    local i len
    for i in "${!tools[@]}"; do
        len=${#tools[$i]}
        [ "$len" -gt "$name_width" ] && name_width=$len
        len=${#versions[$i]}
        [ "$len" -gt "$version_width" ] && version_width=$len
    done
    name_width=$((name_width + 2))
    version_width=$((version_width + 2))

    printf "%-${name_width}s%-${version_width}s%s\n" "TOOL/STEP" "VERSION" "STATUS"
    printf '%s\n' "$(printf -- '-%.0s' $(seq 1 $((name_width + version_width + 12))))"

    local failed=0 warning=0 t s v color

    for i in "${!tools[@]}"; do
        t="${tools[$i]}"
        s="${statuses[$i]}"
        v="${versions[$i]}"
        [ -z "$v" ] && v="-"
        color="$CV_COLOR_WHITE"
        case "$s" in
            OK|Installed|Upgraded) color="$CV_COLOR_GREEN" ;;
            Warning) color="$CV_COLOR_YELLOW"; warning=$((warning + 1)) ;;
            DryRun) color="$CV_COLOR_CYAN" ;;
            Skipped) color="$CV_COLOR_GRAY" ;;
            Failed) color="$CV_COLOR_RED"; failed=$((failed + 1)) ;;
        esac
        printf "%-${name_width}s%-${version_width}s" "$t" "$v"
        printf "%b%s%b\n" "$color" "$s" "$CV_COLOR_RESET"
    done

    printf '%s\n\n' "$(printf -- '-%.0s' $(seq 1 $((name_width + version_width + 12))))"

    if [ "$failed" -gt 0 ]; then
        log_error "Bootstrap finished with $failed failure(s)."
    elif [ "$warning" -gt 0 ]; then
        log_warning "Bootstrap finished with $warning warning(s)."
    else
        log_success "Bootstrap finished successfully."
    fi

    CV_SUMMARY_FAILED=$failed
    CV_SUMMARY_WARNING=$warning
}

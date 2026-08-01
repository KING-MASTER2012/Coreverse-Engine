#!/usr/bin/env bash
# Coreverse Bootstrap - dependency-aware parallel task runner.
#
# Independent tasks run concurrently as background jobs; a task only starts
# once every task listed in its DependsOn has finished. Mirrors the
# Invoke-TaskGraph design from the PowerShell (Windows) implementation.
# Sourced by every script that needs it; do not execute directly.
#
# Each task spec is a single string: "Name|/absolute/path/to/script.sh|args|Dep1,Dep2"
#   - Name must not contain '|', ',', or whitespace.
#   - args is a plain space-separated argument string (no embedded quoting needed
#     for this project's simple values).
#   - Dep1,Dep2 is a comma-separated list of task Names this task depends on
#     (empty string if none).

# run_task_graph <results_dir> <task_spec...>
# Writes one <results_dir>/logs/<Name>.log (captured stdout+stderr, replayed in
# order after each layer) per task. Each check-*.sh/parse-*.sh script is
# responsible for writing its own <results_dir>/<Name>.result file (see
# tool-check-helper.sh's write_result).
run_task_graph() {
    local results_dir="$1"
    shift
    local tasks=("$@")
    mkdir -p "$results_dir/logs"

    declare -A completed
    local remaining=("${tasks[@]}")

    while [ "${#remaining[@]}" -gt 0 ]; do
        local runnable=()
        local still_remaining=()
        local task name deps all_met dep_arr d

        for task in "${remaining[@]}"; do
            deps=$(printf '%s' "$task" | awk -F'|' '{print $4}')
            all_met=1
            if [ -n "$deps" ]; then
                IFS=',' read -ra dep_arr <<< "$deps"
                for d in "${dep_arr[@]}"; do
                    if [ -z "${completed[$d]:-}" ]; then
                        all_met=0
                        break
                    fi
                done
            fi
            if [ "$all_met" -eq 1 ]; then
                runnable+=("$task")
            else
                still_remaining+=("$task")
            fi
        done

        if [ "${#runnable[@]}" -eq 0 ]; then
            log_error "Unresolved or circular dependency among remaining tasks." "TaskGraph"
            return 1
        fi

        local pids=() names=()
        local script args

        for task in "${runnable[@]}"; do
            name=$(printf '%s' "$task" | awk -F'|' '{print $1}')
            script=$(printf '%s' "$task" | awk -F'|' '{print $2}')
            args=$(printf '%s' "$task" | awk -F'|' '{print $3}')

            (
                # shellcheck disable=SC2086
                "$script" $args --result-file "$results_dir/$name.result"
            ) > "$results_dir/logs/$name.log" 2>&1 &

            pids+=("$!")
            names+=("$name")
        done

        local pid
        for pid in "${pids[@]}"; do
            wait "$pid"
        done

        for name in "${names[@]}"; do
            cat "$results_dir/logs/$name.log"
            completed["$name"]=1
        done

        remaining=("${still_remaining[@]}")
    done
}

# run_parallel_tasks <results_dir> <task_spec...>
# Simpler variant for tasks with no dependencies between them (all run in one layer).
run_parallel_tasks() {
    local results_dir="$1"
    shift
    local tasks=("$@")
    local tasks_with_no_deps=()
    local task

    for task in "${tasks[@]}"; do
        tasks_with_no_deps+=("${task}|")
    done

    run_task_graph "$results_dir" "${tasks_with_no_deps[@]}"
}

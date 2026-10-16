set -u
OUT=${OUT:?set OUT to an output dir}; TEST_BIN=${TEST_BIN:?set TEST_BIN to the built live-* test binary}
PROBE_DIR=$OUT $TEST_BIN default_term_dies --exact --nocapture >$OUT/test.out 2>&1 &
TP=$!
sleep 1
echo "test_pid=$TP"
for t in /proc/$TP/task/[0-9]*; do echo "$(basename $t) comm=$(cat $t/comm) state=$(awk '{print $3}' $t/stat) wchan=$(cat $t/wchan)"; done
cat /proc/$TP/task/$TP/stack >/dev/null 2>$OUT/stack.err && echo stack_readable=yes || echo "stack_readable=no ($(cat $OUT/stack.err))"
cat /proc/$TP/syscall >/dev/null 2>&1 && echo "syscall_readable=yes: $(cat /proc/$TP/syscall | cut -c1-40)" || echo syscall_readable=no
s=$(date +%s.%N); gdb -q -batch -nx -p $TP -ex 'set pagination off' -ex 'thread apply all bt' >$OUT/gdb.txt 2>&1; e=$(date +%s.%N); echo "gdb_rc=$? gdb_seconds=$(echo "$e - $s" | bc)"
kill -0 $TP && echo test_alive_after_gdb=yes
s=$(date +%s.%N); perf record -q -F 99 --call-graph dwarf -p $TP -o $OUT/perf.data -- sleep 1 >$OUT/perf.rec.txt 2>&1; echo "perf_rc=$? perf_seconds=$(echo "$(date +%s.%N) - $s" | bc)"
perf report -i $OUT/perf.data --stdio --no-children 2>/dev/null | grep -v '^#' | grep -v '^$' | head -15 > $OUT/perf.report.txt
kill -TERM $TP; wait $TP; echo "test_exit=$?"

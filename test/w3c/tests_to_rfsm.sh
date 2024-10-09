#/bin/bash

echo "Started from $(pwd)"
SCRIPT_DIR="$(dirname "$(readlink -f "$0")")"
cd $SCRIPT_DIR
echo "Working in $(pwd)"

TOFSM_BIN=../../target/debug/scxml_to_fsm

echo "======================================================="

export RUST_LOG=debug
export RUST_BACKTRACE=full

mkdir rfsm

for TEST_FILE in scxml/*.scxml; do

  TEST_NAME=$(basename "${TEST_FILE}")

  $TOFSM_BIN -includePaths dependencies/scxml "$TEST_FILE" rfsm/$TEST_NAME.rfsm

done


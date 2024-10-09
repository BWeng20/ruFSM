#/bin/bash
# Converts all SCXML files from "scxml" to "rfms" files.
# "RFSM" is the binary format written by the rfsm serializer.
# See also "execute_all_rfsm.sh".

echo "Started from $(pwd)"
SCRIPT_DIR="$(dirname "$(readlink -f "$0")")"
cd $SCRIPT_DIR
echo "Working in $(pwd)"

TOFSM_BIN=../../target/release/scxml_to_fsm

echo "======================================================="

export RUST_LOG=debug
export RUST_BACKTRACE=full

if [ -d rfsm ]; then
  echo "Cleaning rfsm"
  rm -rf rfsm/*
fi
mkdir rfsm

for TEST_FILE in scxml/*.scxml; do

  TEST_NAME=$(basename "${TEST_FILE}")

  $TOFSM_BIN -includePaths dependencies/scxml "$TEST_FILE" rfsm/$TEST_NAME.rfsm

done


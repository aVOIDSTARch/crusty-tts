#!/usr/bin/env bash
# sample-tts/run.sh - reads PLUGIN_INPUT, outputs mock audio (input bytes) for testing
INPUT="${PLUGIN_INPUT:-Hello world}"
# Echo input as "audio" for pipeline test (no real TTS required)
printf '%s' "$INPUT"

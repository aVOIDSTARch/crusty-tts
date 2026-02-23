#!/usr/bin/env python3
"""
Minimal Crusty-TTS plugin: reads PLUGIN_INPUT from env, writes raw bytes to stdout.
Implements the same contract as run.sh plugins (env-based, no framing).
"""
import os
import sys

def main():
    input_text = os.environ.get("PLUGIN_INPUT", "")
    voice = os.environ.get("PLUGIN_OPT_VOICE", "en_us")
    # Stub: output input as "audio" for pipeline test (no real TTS)
    sys.stdout.buffer.write(input_text.encode("utf-8"))
    sys.stdout.buffer.flush()

if __name__ == "__main__":
    main()

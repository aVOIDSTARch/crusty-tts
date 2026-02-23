# Crusty-TTS pipeline example

[meta]
name = "demo-pipeline"
version = "0.1.0"
author = "Louie"

[input]
type = "text"
source = "input.txt"

[tts]
name = "sample-tts"
module = "plugins/sample-tts"
voice = "en_us"
rate = 1.0
output_format = "wav"

[output]
type = "file"
path = "output/out.bin"
overwrite = true

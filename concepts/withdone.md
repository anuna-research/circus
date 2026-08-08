# withdone

`withdone` is a local wrapper that waits for an agent to write a caller-supplied
sentinel file. It then ends the wrapped process group; Circus uses this explicit
signal instead of treating an agent exit code as acceptance.

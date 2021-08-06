# gst-launch-1.0 shell completion
### Get suggestions for elements and it's properties as you build your gstreamer pipeline in the bash 🚀

[![demo](example.svg)](example.svg)

- Suggests/Autocompletes next compatible elements in the pipeline.
- Suggests/Autocompletes properties of the current element


### Build and Install
`cargo build --release && cargo install --path .` 

The executable called `_gst_completion` will be installed in   `$HOME/.cargo/bin`  or wherever your cargo path is setup. Make sure this path is in your `$PATH`.

### Setup
`complete -o nosort -C _gst_completion gst-launch-1.0`

That's it and you are good to go!





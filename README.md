# rtmp2many
Stream a video file to multiple RTMP publish points using GStreamer bindings for Rust

## Methodology
1. Create multiple RTMP publish points using nginx. Download the software and the config files from [here](https://djp.li/rtmpstreaming)
2. Extract the downloaded files in _root/nginx_, in my case _C:/nginx_
3. Modify the _nginx/conf/nginx.conf_ to create multiple applications which are going to server as multiple publish points for RTMP streams
  * sublish 

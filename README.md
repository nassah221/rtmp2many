# rtmp2many
Stream a video file to multiple RTMP publish points using GStreamer bindings for Rust

## Creating Multiple RTMP Endpoints with nginx
1. Create multiple RTMP publish points using nginx. Download the software and the config files from [here](https://djp.li/rtmpstreaming)
2. Extract the downloaded files in _root/nginx_, in my case _C:/nginx_
3. Modify the _nginx/conf/nginx.conf_ to create multiple applications which are going to server as multiple publish points for RTMP streams
  * [logo]: https://github.com/nassah221/rtmp2many/blob/main/assets/nginx_conf.png "Multiple RTMP endpoints"
4. Make a copy of _viewer.html_ and change the _stream_key_. This is to be done for every application created in the _conf file_. In our case the stream keys are _stream1_ & _stream2_
5. Having created multiple endpoints _live1_ & _live2_, start the nginx server by executing _start.bat_

## Publishing to the Endpoints
`cargo r -- -u rtmp://127.0.0.1/live1/stream1 rtmp://127.0.0.1/live2/stream2 -f "path:\\to\\video\\file"`

## Streaming from the Endpoints
Use `gst-launch` or VLC Media Player to stream the video
 * `gst-launch-1.0 uridecodebin uri="rtmp://127.0.0.1/live1/stream1" name=src ! queue ! videoconvert ! autovideosink src. ! queue ! audioconvert ! audioresample ! autoaudiosink`
 * `gst-launch-1.0 uridecodebin uri="rtmp://127.0.0.1/live2/stream2" name=src ! queue ! videoconvert ! autovideosink src. ! queue ! audioconvert ! audioresample ! autoaudiosink`

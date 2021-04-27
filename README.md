# fileupload-rs
Upload file and Generate a random name to download file. File expires in 1 hour(configurable)

There are sample keys for HTTPS in config directory

Generate your own
`openssl req -x509 -newkey rsa:4096 -nodes -keyout key.pem -out cert.pem -days 365 -subj '/CN=localhost'`


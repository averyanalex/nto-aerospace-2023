stty -F /dev/ttyACM0 raw
pppd /dev/ttyACM0 115200 10.0.5.1:10.0.5.2 proxyarp local noauth debug nodetach dump nocrtscts passive persist maxfail 0 holdoff 1

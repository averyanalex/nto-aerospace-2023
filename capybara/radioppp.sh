stty -F /dev/ttyUSB0 raw
pppd /dev/ttyUSB0 57600 10.0.5.2:10.0.5.1 proxyarp local noauth debug nodetach dump nocrtscts passive persist maxfail 0 holdoff 1

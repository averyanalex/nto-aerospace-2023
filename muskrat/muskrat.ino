#define SERVO_PIN 44
#define BUTTON_PIN A8

#include <Servo.h>

Servo claw;

void setup() {
  pinMode(BUTTON_PIN, INPUT_PULLUP);
  claw.attach(SERVO_PIN);
  Serial.begin(115200);
}

union ArrayToInteger {
  byte array[4];
  uint32_t integer;
};

void loop() {
  if (!digitalRead(BUTTON_PIN)) {
    while (!digitalRead(BUTTON_PIN)) {
      delay(100);
    }
    Serial.write(0x07);
  }

  if (Serial.available()) {
    byte buf[5];
    Serial.readBytes(buf, 5);
    ArrayToInteger converter = {buf[1], buf[2], buf[3], buf[4]};
    if (buf[0] == 0x3) {
      claw.writeMicroseconds(converter.integer);
    }
  }
}

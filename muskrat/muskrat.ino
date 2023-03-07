#define SERVO_PIN 44
#define BUTTON_PIN A8

#include <Servo.h>

Servo claw;

void setup() {
  pinMode(BUTTON_PIN, INPUT_PULLUP);
  claw.attach(SERVO_PIN);
  Serial.begin(115200);
}


void loop() {
  if (!digitalRead(BUTTON_PIN)) {
    while (!digitalRead(BUTTON_PIN)) {
      delay(100);
    }
    Serial.write(0x07);
  }

  if (Serial.available()) {
//    int angle = Serial.parseInt();
//    Serial.println(angle);
//    if (angle != 0) {
//      claw.writeMicroseconds(angle);
//    }
  }
}

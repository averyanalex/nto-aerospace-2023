#define SERVO_PIN 44
#define BUTTON_PIN A8

#include <Servo.h>

Servo claw;
bool up = false;

void setup() {
  pinMode(BUTTON_PIN, INPUT_PULLUP);
  claw.attach(SERVO_PIN);
}


void loop() {
  if (!digitalRead(BUTTON_PIN)) {
    while (!digitalRead(BUTTON_PIN)) {
      delay(100);
    }
    up = !up;
  }

  if (up) {
    claw.write(90);
  } else {
    claw.write(0);
  }
}

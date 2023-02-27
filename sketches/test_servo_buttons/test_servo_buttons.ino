#define SERVO_PIN 0
#define BUTTON_PIN_0 1
#define BUTTON_PIN_1 2
#define BUTTON_PIN_2 3

#include <Servo.h>

Servo claw;

void setup() {
  pinMode(BUTTON_PIN_0, INPUT_PULLUP);
  pinMode(BUTTON_PIN_1, INPUT_PULLUP);
  pinMode(BUTTON_PIN_2, INPUT_PULLUP);
  claw.attach(SERVO_PIN);
}

void loop() {
  if (!digitalRead(BUTTON_PIN_0)) {
    claw.write(0);
    delay(50);
  }
  if (!digitalRead(BUTTON_PIN_1)) {
    claw.write(120);
    delay(50);
  }
  if (!digitalRead(BUTTON_PIN_2)) {
    claw.write(60);
    delay(50);
  }
}

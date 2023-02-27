#define SERVO_PIN 3
#define BUTTON_PIN 8

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
    Serial.println(1);
  }

  if (Serial.available()) {
    int angle = Serial.parseInt();
    if (angle != 0) {
      claw.write(angle);
    }
  }
}

RLC Series Resonant Circuit
* Demonstrates RLC series resonance
* Resonant frequency: f0 = 1/(2*pi*sqrt(LC)) ≈ 159 kHz
* Quality factor: Q = (1/R)*sqrt(L/C) = 10

V1 1 0 DC 0V PULSE(0V 10V 0s 1ns 1ns 100ns 1us)
R1 1 2 100
L1 2 3 1u
C1 3 0 1n

.tran 10ns 20us
.end 
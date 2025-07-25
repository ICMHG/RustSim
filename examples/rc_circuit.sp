RC Low-Pass Filter Circuit
* Demonstrates transient analysis of RC circuit
* Time constant: tau = R*C = 1k * 1n = 1us
* 3dB frequency: f = 1/(2*pi*tau) ≈ 159 kHz

V1 1 0 DC 0V PULSE(0V 5V 0s 1ns 1ns 500ns 1us)
R1 1 2 1k
C1 2 0 1n

.tran 10ns 5us
.end 
DC Sweep Analysis Example
* Voltage divider with variable input voltage
* Demonstrates linear relationship between input and output
* V_out = V_in * R2/(R1+R2) = V_in * 2k/(1k+2k) = V_in * 2/3

V1 1 0 DC 0V
R1 1 2 1k
R2 2 0 2k

.dc V1 0V 5V 0.5V
.end 
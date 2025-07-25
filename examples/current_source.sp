Current Source Circuit
* Demonstrates current source with parallel resistors
* Current divides between R1 and R2 according to their resistance values
* I_R1 = I1 * R2/(R1+R2) = 1mA * 2k/(1k+2k) = 0.667mA
* I_R2 = I1 * R1/(R1+R2) = 1mA * 1k/(1k+2k) = 0.333mA

I1 0 1 DC 1mA
R1 1 0 1k
R2 1 0 2k

.op
.end 
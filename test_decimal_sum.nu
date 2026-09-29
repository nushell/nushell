# Test decimal math sum functionality
# This script tests the decimal math sum feature

# Test basic decimal sum
echo "Testing decimal sum..."
[1.1, 2.2, 3.3] | math sum

# Test mixed int and decimal sum  
echo "Testing mixed int and decimal sum..."
[1, 2.5, 3] | math sum

# Test large decimal sum
echo "Testing large decimal sum..."
[999999999.999999999, 0.000000001] | math sum

# Test decimal sum with zero
echo "Testing decimal sum with zero..."
[0.0, 1.5, 2.5] | math sum

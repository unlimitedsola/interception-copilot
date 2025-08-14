#!/bin/bash
echo "Testing argument parsing..."
echo "No args:" 
echo "args: 0" | cargo run --target x86_64-pc-windows-gnu --example escape_blocker --quiet 2>&1 || echo "Expected: would normally run the full program"
echo "With valid precedence:"
echo "args: 5" | cargo run --target x86_64-pc-windows-gnu --example escape_blocker --quiet -- 5 2>&1 || echo "Expected: would normally run the full program"
echo "With invalid precedence:"  
echo "args: abc" | cargo run --target x86_64-pc-windows-gnu --example escape_blocker --quiet -- abc 2>&1 || echo "Expected: would normally run the full program"


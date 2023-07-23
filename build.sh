#!/bin/bash

forge clean

forge install foundry-rs/forge-std --no-commit
forge install transmissions11/solmate@v7 --no-commit
forge install primitivefinance/portfolio@6bdd71a0844f3587bca96e955fd336f906c82140 --no-commit

forge bind --crate-name bindings --overwrite

echo "Completed build shell script"
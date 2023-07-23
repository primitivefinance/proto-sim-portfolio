#!/bin/bash

forge clean

forge install foundry-rs/forge-std --no-commit
forge install transmissions11/solmate@v7 --no-commit
forge install primitivefinance/portfolio@2978df260796c77bfdbc2c515e8187e2fef36af7 --no-commit

forge bind --crate-name bindings --overwrite

echo "Completed build shell script"
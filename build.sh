#!/bin/bash

forge clean

forge install foundry-rs/forge-std --no-commit
forge install transmissions11/solmate@v7 --no-commit
forge install primitivefinance/portfolio@f8302e0e9d406c70dfd5178157f75bbd8d3a21de --no-commit

forge bind --crate-name bindings --overwrite --via-ir

echo "Completed build shell script"

#!/bin/bash

forge clean

forge install foundry-rs/forge-std --no-commit
forge install transmissions11/solmate@v7 --no-commit
forge install primitivefinance/portfolio@32471841a2331c0a6305dfa286b81d279156ed2f --no-commit

forge bind --crate-name bindings --overwrite

echo "Completed build shell script"
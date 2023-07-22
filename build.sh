#!/bin/bash

forge clean

forge install foundry-rs/forge-std --no-commit
forge install transmissions11/solmate@v7 --no-commit
forge install https://github.com/primitivefinance/portfolio@1b2aa982c0eb10773219602f19bdf7323bfe2a62 --no-commit

forge bind --crate-name bindings --overwrite

echo "Completed build shell script"
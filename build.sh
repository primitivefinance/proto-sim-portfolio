#!/bin/bash

forge clean

forge install foundry-rs/forge-std --no-commit
forge install transmissions11/solmate@v7 --no-commit
forge install https://github.com/primitivefinance/portfolio@3e2ed512790db33088427a4611f69f4849ede95b --no-commit

forge bind --crate-name bindings --overwrite

echo "Completed build shell script"
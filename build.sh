#!/bin/bash

forge clean

forge install foundry-rs/forge-std --no-commit
forge install transmissions11/solmate@e8f96f25d48fe702117ce76c79228ca4f20206cb --no-commit
forge install primitivefinance/portfolio@728b04f29c1e66875d5fdac7e24cd0422ad17caa --no-commit

forge bind --crate-name bindings --overwrite --via-ir --force

echo "Completed build shell script"

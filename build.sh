#!/bin/bash

forge clean

forge install

forge bind --crate-name bindings --overwrite

echo "Completed build shell script"
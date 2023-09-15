#!/bin/bash

# Check if 'seeds' directory exists. If not, create one.
if [ ! -d "seeds" ]; then
    mkdir seeds
fi

# Find all "*.nu" files from '../..' excluding the 'seeds' directory and copy them into 'seeds' directory.
find ../../.. -type f -name "*.nu" -exec cp {} seeds/ \;


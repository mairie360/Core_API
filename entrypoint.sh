#!/bin/bash

cargo watch \
  -w src \
  -w api_lib \
  -i target \
  -i api_lib/target \
  -x run

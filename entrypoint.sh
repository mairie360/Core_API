#!/bin/bash

cargo watch \
  -w src \
  -w api_lib \
  -w api_macro_lib \
  -i target \
  -i api_lib/target \
  -i api_macro_lib/target \
  -x run

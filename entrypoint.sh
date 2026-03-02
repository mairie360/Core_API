#!/bin/bash

cargo watch \
  -w src \
  -i target \
  -x run

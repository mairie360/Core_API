#!/bin/bash
set -e

# CORRECTION : Utiliser le même dossier que le WORKDIR du Dockerfile
cd /usr/src/core

# Lancer cargo watch
exec cargo watch -w src -i target -x run
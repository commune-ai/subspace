#!/bin/bash

# Create the docker group if it doesn't already exist
sudo groupadd docker || true

# Add the current user to the docker group
sudo usermod -aG docker $USER

# Activate the changes to the current session
newgrp docker

# Verify that the docker group membership is applied correctly
docker run hello-world

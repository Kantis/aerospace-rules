#!/bin/bash

# Aerospace Rules Service Restart Script

echo "Stopping aerospace-rules service..."
pkill -f aerospace-rules-service

echo "Waiting for service to stop..."
sleep 1

echo "Starting aerospace-rules service..."
./target/release/aerospace-rules-service 1>/dev/null &

echo "Service restarted with PID $!"
echo "Use 'pkill -f aerospace-rules-service' to stop it"

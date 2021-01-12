#!/bin/bash -u

. ./.env

echo "Starting elk..."
docker-compose -f docker-compose_elk.yml up --detach

#echo "Starting mongodb..."
#docker-compose -f docker-compose_mongo.yml up --detach

#!/bin/bash -u

. ./.env

echo "Resuming elk..."
docker-compose -f docker-compose_elk.yml start

echo "Resuming mongodb..."
docker-compose -f docker-compose_mongo.yml start

cd /home/mrj/MrJ/Projects/Besu/besu-sample-networks || exit
. ./resume.sh


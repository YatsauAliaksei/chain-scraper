#!/bin/bash -u

echo "Stop and remove db..."
docker-compose -f docker-compose_db.yml down -v
docker-compose -f docker-compose_db.yml rm -sfv

echo "Stop and remove elk..."
docker-compose -f docker-compose_elk.yml down -v
docker-compose -f docker-compose_elk.yml rm -sfv

echo "Stop and remove mongo..."
docker-compose -f docker-compose_mongo.yml down -v
docker-compose -f docker-compose_mongo.yml rm -sfv

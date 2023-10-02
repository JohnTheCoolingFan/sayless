reset:
    sqlx database drop -y
    cargo clean
    sqlx database setup -y
    cargo build
    cargo sqlx prepare

docker-build:
    docker build -t sayless:1.0.0 ./

docker-run: docker-build
    docker run --network=host -v `pwd`/.env:/sayless/.env sayless:1.0.0

docker-clean:
    docker image prune

FROM rust:latest as build


COPY ./ ./

# proxy setting
ENV https_proxy=http://192.168.2.24:6152 \
    http_proxy=http://192.168.2.24:6152;
RUN cargo build --release

RUN mkdir -p /build-out

RUN cp target/release/factotum-server /build-out/
RUN cp factotum /build-out/

FROM ubuntu:18.04

RUN apt-get update && apt-get -y install ca-certificates libssl-dev 

COPY --from=build /build-out/factotum-server /
COPY --from=build /build-out/factotum /

CMD /factotum-server --factotum-bin=/factotum

FROM ubuntu:20.10

# Install required dependencies
RUN apt-get update &&  apt-get upgrade -y && apt-get install -y openssl

COPY clover /opt/clover/bin/clover
COPY specs /opt/specs

WORKDIR /opt/clover
CMD /opt/clover/bin/clover $ARGS

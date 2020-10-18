FROM ubuntu:20.10

# Install required dependencies
RUN apt-get update &&  apt-get upgrade -y && apt-get install -y openssl

ENV HOME /home/clover
RUN adduser --quiet --disabled-password --shell /bin/bash --home $HOME --gecos '' clover && \
    echo "clover:password" | chpasswd && \
    adduser clover sudo


COPY clover /opt/clover/bin/clover

USER clover

WORKDIR /opt/clover
CMD /opt/clover/bin/clover $ARGS
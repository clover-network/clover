FROM cloverio/runtime-linux-production:bullseye

# Install required dependencies
RUN apt-get update && apt-get install ca-certificates  -y

COPY clover /opt/clover/bin/clover
COPY specs /opt/specs

WORKDIR /opt/clover
CMD /opt/clover/bin/clover $ARGS

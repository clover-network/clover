FROM cloverio/runtime-linux-production:bullseye

COPY clover /opt/clover/bin/clover
COPY specs /opt/specs

WORKDIR /opt/clover
CMD /opt/clover/bin/clover $ARGS

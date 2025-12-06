# Use Alpine Linux as a base
FROM alpine:3

# Install the necessary build tools
RUN apk add --no-cache cargo cmake ninja clang clang-libclang
ARG CMAKE_GENERATOR="Ninja"
ARG C="clang"
ARG CXX="clang++"

# Begin setting up the build folder
COPY --exclude=target/ . /opt/kawari-build
WORKDIR /opt/kawari-build

# Build for release
RUN cargo build --release

# Copy binaries
RUN mkdir /opt/kawari
RUN cp /opt/kawari-build/target/release/kawari-* /opt/kawari

# Clean up the build folder, it's no longer needed
RUN rm -rf /opt/kawari-build

WORKDIR /opt/kawari
CMD /opt/kawari/kawari-run

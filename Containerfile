# Use Alpine Linux as a base
FROM alpine:3

# Install the necessary build tools
RUN apk add --no-cache cargo cmake ninja clang clang-libclang
ARG CMAKE_GENERATOR="Ninja"
ARG C="clang"
ARG CXX="clang++"

# Begin setting up the build folder
COPY . /opt/kawari-build
WORKDIR /opt/kawari-build

# Build for release
RUN cargo build --release

CMD ["kawari-world"]

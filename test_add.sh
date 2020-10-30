#!/usr/bin/env bash

export CNI_COMMAND=ADD
export CNI_CONTAINERID=$(uuidgen)
export CNI_NETNS=ns
export CNI_IFNAME=mvlandefault
export CNI_PATH=/opt/cni/bin

export RUST_BACKTRACE=1

cargo run --package cni-consul-ipam --bin consul < ./testconfig.json
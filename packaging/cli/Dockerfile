# INVARIANT: Only the generated private rootfs enters this image; no registry base.
FROM scratch
COPY rootfs/ /
USER 65532:65532
WORKDIR /
ENTRYPOINT ["/usr/local/bin/triage-mail-cli"]
CMD ["--help"]

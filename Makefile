.PHONY: all build audit trivy security

all: build

build:
	cargo build

audit:
	cargo audit

trivy:
	docker build -t ai-gateway:security .
	docker run --rm -v /var/run/docker.sock:/var/run/docker.sock aquasec/trivy:latest image --exit-code 1 --format table ai-gateway:security

security: audit trivy

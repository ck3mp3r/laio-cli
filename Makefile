act-test-release:
	@act workflow_dispatch \
		--rm \
		# --container-architecture linux/amd64 \
		-s GITHUB_TOKEN=${GITHUB_TOKEN} \
		-s ACTIONS_RUNTIME_TOKEN=${GITHUB_TOKEN} \
		-P ubuntu-latest=catthehacker/ubuntu:js-latest \
		-W .github/workflows/release.yaml

act-test:
	act push \
    --pull=false \
		--container-architecture linux/arm64 \
		-s GITHUB_TOKEN=${GITHUB_TOKEN} \
		-s ACTIONS_RUNTIME_TOKEN=${GITHUB_TOKEN} \
		-P ubuntu-latest=catthehacker/ubuntu:act-latest \
		-W .github/workflows/test.yaml \
		-j test

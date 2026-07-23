package github

#ActionsWorkflowRef: {
	in: {
		// Repository in OWNER/REPOSITORY form, or an expression resolving to it.
		repository: string & !=""

		// Names a file directly under .github/workflows.
		workflowFile: =~"^[^/@]+\\.ya?ml$"

		// Fully qualified branch or tag ref, or a commit SHA.
		gitRef: string & !=""
	}

	val: "\(in.repository)/.github/workflows/\(in.workflowFile)@\(in.gitRef)"
}

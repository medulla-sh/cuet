@if(test)

package google

#GkeKubernetesServiceAccountPrincipalTests: {
	"current-environment": {
		input: #GkeKubernetesServiceAccountPrincipal & {"in": {
			namespace:      "flux-system"
			serviceAccount: "flux"
		}}
		input: out: #envName: "dev"

		assert: input.out.data.google_project.dev == {}
		assert: input.val == "principal://iam.googleapis.com/projects/${data.google_project.dev.number}/locations/global/workloadIdentityPools/${data.google_project.dev.project_id}.svc.id.goog/subject/ns/flux-system/sa/flux"
	}

	"id-only": {
		input: #GkeKubernetesServiceAccountPrincipal & {"in": {
			namespace:      "flux-system"
			serviceAccount: "flux"
			project: id: "oakmont-dev"
		}}

		assert: input.out.data.google_project["oakmont-dev"].project_id == "oakmont-dev"
		assert: input.val == "principal://iam.googleapis.com/projects/${data.google_project.oakmont-dev.number}/locations/global/workloadIdentityPools/oakmont-dev.svc.id.goog/subject/ns/flux-system/sa/flux"
	}
}

gkeResult: [for _, test in #GkeKubernetesServiceAccountPrincipalTests {test.assert & true}]

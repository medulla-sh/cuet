@if(test)

package kubernetes

#DataServiceTests: {
	"provider-alias": {
		input: #DataService & {in: {
			#providerAlias: "dev"
			serviceName:    "public-istio"
			namespace:      "gateway-system"
		}}

		assert: input.out.data.kubernetes_service_v1["gateway-system-public-istio"].#providerAlias == "dev"
	}
}

dataServiceResult: [for _, test in #DataServiceTests {test.assert & true}]

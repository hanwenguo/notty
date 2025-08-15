#let config = (
  name: "Site Name",
  description: "Site Description",
  base-url: "https://example.com",
  root-path: "/forest", // use "" if you want to serve from the root
  lang: "en",
  default-author: (
    name: "John Doe",
    id: "john-doe",
  ),
)

#let id-to-path-map = json("/typ/id_path_map.json")

#let id-to-path(id) = {
  id-to-path-map.at(id)
}

#let html-metadata-taxon-map = (
  :
)

#let paged-metadata-taxon-map = (
  :
)

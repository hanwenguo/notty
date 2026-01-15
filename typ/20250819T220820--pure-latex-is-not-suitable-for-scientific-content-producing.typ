#import "/_template/template.typ": template, tr, ln
#show: template(
  title:      [Pure LaTeX is not suitable for scientific content producing],
  date:       datetime(year: 2025, month: 08, day: 19, hour: 22, minute: 08, second: 20),
  tags:       (),
  author: ("hanwenguo",),
  identifier: "20250819T220820",
)

Many might argue against this, but I think that pure LaTeX, that is, just LaTeX itself, is not suitable for scientific content producing. What I agree is that LaTeX is suitable for serious scientific content _publishing_. In that scenario, fine-grained control over typesetting is important, and following the popular standard and utilizing the ecosystem is also important. LaTeX is probably the best one for that purpose. However, I think there are more stuff besides publishing in scientific content producing --- one needs to effectively take notes of scientific contents, create new scientific contents not only for publishing (for example, private note, temporary thought, manuscript, etc.), and manage these contents. And the following disadvantages prevent LaTeX to be suitable for these purpose: its compilation is too slow, thus one cannot preview their input and reflect upon it immediately; its language is far from modern, thus it is hard for one to be able to grasp it so that they can build their own utilities on it; also, it is too heavy to be integrated into other tools.

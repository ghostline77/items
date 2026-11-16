# Preview recipes

Recipes describe the feature being compared, its native reference assemblies and
the views that make the feature legible. Family labels alone do not determine
framing. Native customization ownership selects the male or female reference
assemblies; mesh filenames do not establish compatibility.

| Feature | Context and framing | Views |
| --- | --- | --- |
| Hair | Neutral native head; fit the complete hairstyle | Three-quarter, front, rear |
| Nail polish and patterns | Neutral native arms/hands; fit one hand's nail component | Three-quarter, front, rear |
| Iris, eyebrows, eyelashes | Native head and neutral eyes; fit the subject component | Three-quarter, front |
| Lip, eye and cheek makeup | Owning customization root's native head; fit the overlay | Three-quarter, front |
| Facial tattoos, scars, cyberware | Owning native head and neutral eyes; fit the subject | Front, left, right |
| Piercings | Owning native head and neutral eyes; at least 35 mm of framing radius around the mounted subject | Three-quarter, front, subject-side profile |
| Head skin | Selected native head surface and neutral eyes | Three-quarter, front |
| Teeth | Unobstructed dental geometry; fit the teeth | Three-quarter, front |
| Body skin, tattoos, scars and details | Native neutral body where needed; fit the selected surface | Three-quarter, front, rear |
| Arm surfaces and cyberware | Native neutral body where needed; fit the selected arm geometry | Three-quarter, front, rear |
| Weapons | Isolated native assembly, including a side profile | Three-quarter, left, front |
| Clothing and other equipment | Isolated native assembly fitted to its visible bounds | Three-quarter, front, rear |

Invisible first-person hair/shadow companions and native assemblies with no
enabled visible mesh bindings remain technical catalogue entries without ordinary
3D preview jobs. The plan records the native reason for each omission. Visible
first-person equipment remains renderable. Fresh catalogue builds mark confirmed
nonvisual selections as `not_applicable` rather than pending render work.

`Camera::FrameSubjectPart` fits a single visible native mesh instance and still
draws the reference assemblies. This avoids fitting the distance between both
hands when comparing nail polish. Reference geometry does not expand the detail
camera bounds.

`Camera::FrameSubjectRegion` preserves anatomical context around small mounted
items. Its three-quarter and side views follow the visible subject's side of the
head, and the minimum radius keeps the camera outside the head instead of
magnifying a few-millimetre stud to fill the frame.

The batch runner compares persisted preview settings with the selected recipe.
Changed context, framing, views, material revision or size invalidate the old preview before work
starts. Publication removes unreferenced media. Successful native render receipts
remain in the work directory for validated resume.

Review representative rendered images after changing recipes. A completed GPU
job establishes neither correct presentation nor material fidelity: check native
component selection, placement, reference fit, visible chunks, material/texture
bindings, transparency, lighting and camera before bulk publication.

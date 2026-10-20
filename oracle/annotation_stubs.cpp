// Annotation drawing is OpenGL in the demo; the oracles never reach it.
#include "OpenSteer/Color.h"
#include "OpenSteer/Vec3.h"

namespace OpenSteer {
bool drawPhaseActive = false;
bool enableAnnotation = false;
void drawLine(const Vec3&, const Vec3&, const Color&) {}
void deferredDrawLine(const Vec3&, const Vec3&, const Color&) {}
}

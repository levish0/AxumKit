forbid (principal, action, resource)
when { principal is User && (!principal.active || !principal.verified) };

permit (principal, action in [Action::"read_account", Action::"manage_account"], resource)
when { principal == resource.owner };

permit (principal, action == Action::"read_organization", resource)
when { context.organization_member };

permit (principal, action == Action::"manage_organization", resource)
when { context.organization_member && principal == resource.owner };

permit (principal, action == Action::"administer_account", resource)
when { principal.administrator };

permit (principal, action == Action::"create_post", resource)
when { context.organization_member && principal == resource.owner && !resource.deleted };

permit (principal, action == Action::"read_post", resource)
when { context.organization_member && !resource.deleted &&
    (resource.published || principal == resource.owner || principal == resource.organization_owner) };

permit (principal, action in [Action::"edit_post", Action::"delete_post"], resource)
when { context.organization_member && !resource.deleted &&
    (principal == resource.owner || principal == resource.organization_owner) };

permit (principal, action == Action::"create_comment", resource)
when { context.organization_member && resource.published && !resource.deleted };

permit (principal, action == Action::"delete_comment", resource)
when { context.organization_member && resource.parent_published && !resource.deleted &&
    (principal == resource.owner || principal == resource.post_owner || principal == resource.organization_owner) };

permit (principal, action == Action::"read_external_post", resource)
when { principal.active && principal.organization_id == resource.organization_id && resource.published && !resource.deleted };
